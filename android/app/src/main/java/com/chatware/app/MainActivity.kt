package com.chatware.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.chatware.app.data.Prefs
import com.chatware.app.ui.AppRoot
import com.chatware.app.ui.theme.ChatwareTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val prefs = Prefs(applicationContext)
        setContent {
            ChatwareTheme {
                AppRoot(prefs)
            }
        }
    }
}
