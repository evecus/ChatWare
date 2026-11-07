package com.chatware.app.ui

import android.content.Context
import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewModelScope
import com.chatware.app.data.Prefs
import com.chatware.app.model.ChatMessage
import com.chatware.app.net.ChatApi
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import okhttp3.WebSocket
import org.json.JSONObject
import java.io.File
import java.net.URI

enum class Screen { Auth, ChangePassword, Chat }

class ChatViewModel(private val prefs: Prefs) : ViewModel() {
    val api = ChatApi(prefs)

    var screen by mutableStateOf(
        if (prefs.token.isBlank() || prefs.baseUrl.isBlank()) Screen.Auth else Screen.Chat
    )
        private set

    /** 登录 / 注册切换；注册成功后自动切回登录 */
    var authTab by mutableStateOf(0)

    var messages by mutableStateOf(emptyList<ChatMessage>())
        private set
    var connected by mutableStateOf(false)
        private set
    var busy by mutableStateOf(false)
        private set
    var toast by mutableStateOf<String?>(null)
        private set
    var hasMore by mutableStateOf(true)
        private set

    private var socket: WebSocket? = null
    private var reconnecting = false
    private var everConnected = false

    init {
        if (screen == Screen.Chat) start()
    }

    val myName: String
        get() = prefs.username

    val serverUrl: String
        get() = prefs.baseUrl

    fun showToast(text: String?) {
        toast = text
    }

    /** 校验并保存服务端地址。请求地址每次动态读 prefs，改地址后无需重建客户端。 */
    private fun applyBase(base: String): Boolean {
        val value = base.trim().trimEnd('/')
        val host = runCatching { URI.create(value).host }.getOrNull()
        if (host.isNullOrBlank() || !(value.startsWith("http://") || value.startsWith("https://"))) {
            toast = "请填写完整地址，例如 https://chat.example.com"
            return false
        }
        prefs.baseUrl = value
        return true
    }

    /** 回到登录/注册页（服务端地址也在本页），同时结束旧会话 */
    fun gotoAuth() {
        socket?.cancel()
        socket = null
        connected = false
        everConnected = false
        messages = emptyList()
        prefs.clearSession()
        screen = Screen.Auth
    }

    fun login(base: String, user: String, pass: String) = viewModelScope.launch {
        if (!applyBase(base)) return@launch
        busy = true
        try {
            val result = api.login(user, pass)
            prefs.token = result.token
            prefs.username = result.username
            if (result.mustChangePassword) {
                screen = Screen.ChangePassword
            } else {
                screen = Screen.Chat
                start()
            }
        } catch (e: Exception) {
            toast = e.message
        } finally {
            busy = false
        }
    }

    fun register(base: String, user: String, pass: String, code: String) = viewModelScope.launch {
        if (!applyBase(base)) return@launch
        busy = true
        try {
            api.register(user, pass, code)
            prefs.username = user
            authTab = 0
            toast = "注册成功，请登录"
        } catch (e: Exception) {
            toast = e.message
        } finally {
            busy = false
        }
    }

    fun changePassword(old: String, new: String) = viewModelScope.launch {
        busy = true
        try {
            api.changePassword(old, new)
            screen = Screen.Chat
            start()
        } catch (e: Exception) {
            toast = e.message
        } finally {
            busy = false
        }
    }

    fun logout() = viewModelScope.launch {
        runCatching { api.logout() }
        socket?.cancel()
        socket = null
        connected = false
        everConnected = false
        messages = emptyList()
        prefs.clearSession()
        screen = Screen.Auth
    }

    private fun start() = viewModelScope.launch {
        try {
            prefs.username = api.me()
            loadHistory(reset = true)
            connect()
        } catch (e: Exception) {
            toast = "登录已失效：${e.message}"
            socket = null
            connected = false
            messages = emptyList()
            prefs.clearSession()
            screen = Screen.Auth
        }
    }

    private fun connect() {
        socket?.cancel()
        socket = api.openSocket(
            onMessage = { msg ->
                if (messages.none { it.id == msg.id }) messages = messages + msg
            },
            onError = { err -> toast = err },
            onState = { ok ->
                connected = ok
                if (ok) {
                    if (everConnected) resync()
                    everConnected = true
                } else {
                    scheduleReconnect()
                }
            }
        )
    }

    /** 重连后补拉最新一页，避免断线期间广播的消息丢失 */
    private fun resync() = viewModelScope.launch {
        try {
            val (page, _) = api.messages(null, 50)
            val known = messages.map { it.id }.toHashSet()
            val fresh = page.filter { it.id !in known }
            if (fresh.isNotEmpty()) {
                messages = (messages + fresh).sortedBy { it.key }
            }
        } catch (e: Exception) {
            // 补偿失败不影响已有聊天
        }
    }

    private fun scheduleReconnect() {
        if (reconnecting) return
        reconnecting = true
        viewModelScope.launch {
            delay(2500)
            reconnecting = false
            if (screen == Screen.Chat && prefs.token.isNotBlank()) connect()
        }
    }

    fun loadHistory(reset: Boolean) = viewModelScope.launch {
        try {
            val before = if (reset) null else messages.firstOrNull()?.key
            val (page, more) = api.messages(before)
            messages = if (reset) page else page + messages
            hasMore = more
        } catch (e: Exception) {
            toast = e.message
        }
    }

    fun send(text: String) {
        val value = text.trim()
        if (value.isEmpty()) return
        val ws = socket
        if (ws == null) {
            toast = "连接中，请稍候"
            return
        }
        ws.send(JSONObject().put("type", "text").put("text", value).toString())
    }

    fun uploadFile(context: Context, uri: Uri, name: String) = viewModelScope.launch {
        try {
            toast = "正在上传 $name"
            val meta = api.upload(context, uri, name)
            val ws = socket
            if (ws == null) {
                toast = "文件已上传，但连接未就绪，请稍后重发"
            } else {
                ws.send(JSONObject().put("type", "file").put("file_id", meta.id).toString())
            }
        } catch (e: Exception) {
            toast = e.message
        }
    }

    suspend fun downloadFile(context: Context, id: String, name: String): File =
        api.download(context, id, name)

    override fun onCleared() {
        socket?.cancel()
        socket = null
    }

    companion object {
        fun factory(prefs: Prefs) = object : ViewModelProvider.Factory {
            @Suppress("UNCHECKED_CAST")
            override fun <T : ViewModel> create(modelClass: Class<T>): T =
                ChatViewModel(prefs) as T
        }
    }
}
