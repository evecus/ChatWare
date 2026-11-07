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

    fun clearSession() {
        token = ""
        username = ""
    }
}
