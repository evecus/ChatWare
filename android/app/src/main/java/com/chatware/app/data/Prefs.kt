package com.chatware.app.data

import android.content.Context

class Prefs(context: Context) {
    private val sp = context.getSharedPreferences("chatware", Context.MODE_PRIVATE)

    var baseUrl: String
        get() = sp.getString("base_url", "") ?: ""
        set(value) { sp.edit().putString("base_url", value).apply() }

    var token: String
        get() = sp.getString("token", "") ?: ""
        set(value) { sp.edit().putString("token", value).apply() }

    var username: String
        get() = sp.getString("username", "") ?: ""
        set(value) { sp.edit().putString("username", value).apply() }

    /** 反代证书的 SPKI SHA-256 指纹（base64），留空表示不固定 */
    var fingerprint: String
        get() = sp.getString("fingerprint", "") ?: ""
        set(value) { sp.edit().putString("fingerprint", value).apply() }

    /** 明确开启后跳过 TLS 证书与主机名校验 */
    var skipVerify: Boolean
        get() = sp.getBoolean("skip_verify", false)
        set(value) { sp.edit().putBoolean("skip_verify", value).apply() }

    fun clearSession() {
        token = ""
        username = ""
    }
}
