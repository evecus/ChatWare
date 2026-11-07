package com.chatware.app.net

import android.content.Context
import android.net.Uri
import android.os.Environment
import android.provider.OpenableColumns
import com.chatware.app.data.Prefs
import com.chatware.app.model.ApiException
import com.chatware.app.model.ChatMessage
import com.chatware.app.model.FileMeta
import com.chatware.app.model.LoginResult
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.FormBody
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import okio.BufferedSink
import org.json.JSONObject
import java.io.File
import java.io.FileOutputStream
import java.net.URLEncoder
import java.util.concurrent.TimeUnit

private val JSON_TYPE = "application/json; charset=utf-8".toMediaType()

class ChatApi(private val prefs: Prefs) {
    private val http: OkHttpClient = buildClient()

    val base: String
        get() = prefs.baseUrl.trim().trimEnd('/')

    /** 域名证书由反向代理提供，直接使用系统默认 CA 校验，不做任何证书定制。 */
    private fun buildClient(): OkHttpClient = OkHttpClient.Builder()
        .connectTimeout(15, TimeUnit.SECONDS)
        .readTimeout(90, TimeUnit.SECONDS)
        .writeTimeout(90, TimeUnit.SECONDS)
        .pingInterval(25, TimeUnit.SECONDS)
        .retryOnConnectionFailure(true)
        .build()

    private fun call(path: String, method: String = "GET", body: RequestBody? = null): String {
        val builder = Request.Builder().url(base + path)
        if (prefs.token.isNotBlank()) {
            builder.header("Authorization", "Bearer ${prefs.token}")
        }
        when (method) {
            "POST" -> builder.post(body ?: FormBody.Builder().build())
            "DELETE" -> builder.delete()
            else -> builder.get()
        }
        http.newCall(builder.build()).execute().use { response: Response ->
            val text = response.body?.string().orEmpty()
            if (!response.isSuccessful) {
                val msg = runCatching { JSONObject(text).optString("error") }.getOrNull().orEmpty()
                throw ApiException(msg.ifBlank { "请求失败 ${response.code}" })
            }
            return text
        }
    }

    suspend fun info(): JSONObject = withContext(Dispatchers.IO) {
        JSONObject(call("/api/info"))
    }

    suspend fun login(user: String, pass: String): LoginResult = withContext(Dispatchers.IO) {
        val body = JSONObject().put("username", user).put("password", pass).toString()
            .toRequestBody(JSON_TYPE)
        val o = JSONObject(call("/api/login", "POST", body))
        LoginResult(
            token = o.optString("token"),
            username = o.optString("username")
        )
    }

    suspend fun register(user: String, pass: String, code: String): Unit = withContext(Dispatchers.IO) {
        val body = JSONObject()
            .put("username", user)
            .put("password", pass)
            .put("invite_code", code)
            .toString()
            .toRequestBody(JSON_TYPE)
        call("/api/register", "POST", body)
        Unit
    }

    suspend fun me(): String = withContext(Dispatchers.IO) {
        JSONObject(call("/api/me")).optString("username")
    }

    suspend fun logout(): Unit = withContext(Dispatchers.IO) {
        runCatching { call("/api/logout", "POST") }
        Unit
    }

    suspend fun messages(before: String?, limit: Int = 50): Pair<List<ChatMessage>, Boolean> =
        withContext(Dispatchers.IO) {
            val path = buildString {
                append("/api/messages?limit=").append(limit)
                if (!before.isNullOrBlank()) {
                    append("&before=").append(URLEncoder.encode(before, "UTF-8"))
                }
            }
            val o = JSONObject(call(path))
            val arr = o.optJSONArray("messages")
            val list = ArrayList<ChatMessage>()
            if (arr != null) {
                for (i in 0 until arr.length()) list.add(ChatMessage.from(arr.getJSONObject(i)))
            }
            list to o.optBoolean("has_more")
        }

    fun openSocket(
        onMessage: (ChatMessage) -> Unit,
        onError: (String) -> Unit,
        onState: (Boolean) -> Unit
    ): WebSocket {
        val wsBase = when {
            base.startsWith("https://") -> "wss://" + base.removePrefix("https://")
            base.startsWith("http://") -> "ws://" + base.removePrefix("http://")
            else -> base
        }
        val url = "$wsBase/api/ws?token=" + URLEncoder.encode(prefs.token, "UTF-8")
        val request = Request.Builder().url(url).build()
        return http.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                onState(true)
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                try {
                    val o = JSONObject(text)
                    if (o.optString("type") == "error") {
                        onError(o.optString("error"))
                    } else if (o.has("id")) {
                        onMessage(ChatMessage.from(o))
                    }
                } catch (e: Exception) {
                    onError("消息解析失败")
                }
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                onState(false)
                onError(t.message ?: "连接断开")
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                onState(false)
            }
        })
    }

    suspend fun upload(context: Context, uri: Uri, name: String): FileMeta =
        withContext(Dispatchers.IO) {
            val resolver = context.contentResolver
            val mime = resolver.getType(uri) ?: "application/octet-stream"
            val size = resolver.query(uri, null, null, null, null)?.use { cursor ->
                val idx = cursor.getColumnIndex(OpenableColumns.SIZE)
                if (idx >= 0 && cursor.moveToFirst() && !cursor.isNull(idx)) cursor.getLong(idx) else -1L
            } ?: -1L

            val fileBody = object : RequestBody() {
                override fun contentType() = runCatching { mime.toMediaType() }.getOrNull()
                override fun contentLength(): Long = size
                override fun writeTo(sink: BufferedSink) {
                    val input = resolver.openInputStream(uri)
                        ?: throw java.io.IOException("无法读取所选文件")
                    input.use { stream ->
                        val buffer = ByteArray(64 * 1024)
                        while (true) {
                            val n = stream.read(buffer)
                            if (n < 0) break
                            sink.write(buffer, 0, n)
                        }
                    }
                }
            }
            val multipart = MultipartBody.Builder()
                .setType(MultipartBody.FORM)
                .addFormDataPart("file", name, fileBody)
                .build()
            FileMeta.from(JSONObject(call("/api/files", "POST", multipart)))
        }

    /** 下载到应用外部 Download 目录，返回可直接用 FileProvider 打开的文件 */
    suspend fun download(context: Context, id: String, name: String): File =
        withContext(Dispatchers.IO) {
            val request = Request.Builder()
                .url("$base/api/files/${URLEncoder.encode(id, "UTF-8")}")
                .apply {
                    if (prefs.token.isNotBlank()) {
                        header("Authorization", "Bearer ${prefs.token}")
                    }
                }
                .build()
            http.newCall(request).execute().use { response: Response ->
                if (!response.isSuccessful) throw ApiException("下载失败 ${response.code}")
                val dir = File(
                    context.getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS),
                    "chatware"
                ).apply { mkdirs() }
                val safe = name.replace('/', '_').replace('\\', '_')
                    .replace("\n", "_").replace("\r", "_").ifBlank { "file" }
                val target = File(dir, safe)
                val body = response.body ?: throw ApiException("下载内容为空")
                body.byteStream().use { input ->
                    FileOutputStream(target).use { out -> input.copyTo(out) }
                }
                target
            }
        }
}

fun displayName(context: Context, uri: Uri): String {
    val resolver = context.contentResolver
    return resolver.query(uri, null, null, null, null)?.use { cursor ->
        val idx = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
        if (idx >= 0 && cursor.moveToFirst() && !cursor.isNull(idx)) cursor.getString(idx) else null
    } ?: "shared_file"
}
