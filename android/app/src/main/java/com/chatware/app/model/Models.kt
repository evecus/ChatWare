package com.chatware.app.model

import org.json.JSONObject

class ApiException(message: String) : Exception(message)

data class FileMeta(
    val id: String,
    val name: String,
    val size: Long,
    val mime: String,
    val uploader: String,
    val createdAt: Long
) {
    companion object {
        fun from(o: JSONObject) = FileMeta(
            id = o.optString("id"),
            name = o.optString("name"),
            size = o.optLong("size"),
            mime = o.optString("mime"),
            uploader = o.optString("uploader"),
            createdAt = o.optLong("created_at")
        )
    }
}

data class ChatMessage(
    val id: String,
    val key: String,
    val ts: Long,
    val username: String,
    val kind: String,
    val text: String,
    val file: FileMeta?
) {
    companion object {
        fun from(o: JSONObject): ChatMessage {
            val f = o.optJSONObject("file")
            return ChatMessage(
                id = o.optString("id"),
                key = o.optString("key"),
                ts = o.optLong("ts"),
                username = o.optString("username"),
                kind = o.optString("kind"),
                text = o.optString("text"),
                file = f?.let { FileMeta.from(it) }
            )
        }
    }
}

data class LoginResult(
    val token: String,
    val username: String
)

data class AccountResult(
    val username: String,
    val relogin: Boolean
)
