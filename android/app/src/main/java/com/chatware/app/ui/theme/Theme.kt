package com.chatware.app.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

val Brand = Color(0xFF5B6CFF)
val BrandPurple = Color(0xFF8B5CF6)
val WechatGreen = Color(0xFF95EC69)
val ChatBackground = Color(0xFFEDEDED)

@Composable
fun ChatwareTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = lightColorScheme(
            primary = Brand,
            secondary = BrandPurple,
            background = Color.White,
            surface = Color.White,
            onPrimary = Color.White
        ),
        content = content
    )
}
