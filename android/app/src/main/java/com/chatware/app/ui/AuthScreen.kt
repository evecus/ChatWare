package com.chatware.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.chatware.app.ui.theme.Brand
import com.chatware.app.ui.theme.BrandPurple

@Composable
private fun AuthHeader(subtitle: String) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Box(
            modifier = Modifier
                .size(44.dp)
                .clip(RoundedCornerShape(13.dp))
                .background(Brush.linearGradient(listOf(Brand, BrandPurple))),
            contentAlignment = Alignment.Center
        ) {
            Text("C", color = Color.White, fontWeight = FontWeight.Bold, fontSize = 20.sp)
        }
        Spacer(Modifier.width(12.dp))
        Text("Chatware", fontSize = 20.sp, fontWeight = FontWeight.Bold)
    }
    Text(
        subtitle,
        color = Color.Gray,
        fontSize = 12.sp,
        modifier = Modifier.padding(start = 56.dp, bottom = 20.dp)
    )
}

@Composable
private fun AuthCard(content: @Composable () -> Unit) {
    Box(
        modifier = Modifier.fillMaxSize().background(Color.White),
        contentAlignment = Alignment.Center
    ) {
        Column(
            modifier = Modifier.fillMaxSize().padding(24.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Card(
                modifier = Modifier.fillMaxWidth().widthIn(max = 440.dp),
                shape = RoundedCornerShape(22.dp),
                colors = CardDefaults.cardColors(containerColor = Color.White),
                elevation = CardDefaults.cardElevation(defaultElevation = 8.dp)
            ) {
                Column(modifier = Modifier.padding(24.dp)) { content() }
            }
        }
    }
}

/** 服务端地址 + 登录 / 注册合为同一页；域名证书由反向代理提供，客户端无需证书配置。 */
@Composable
fun AuthScreen(vm: ChatViewModel) {
    var base by remember { mutableStateOf(vm.serverUrl.ifBlank { "https://" }) }
    var user by remember { mutableStateOf(vm.myName) }
    var pass by remember { mutableStateOf("") }
    var pass2 by remember { mutableStateOf("") }
    var code by remember { mutableStateOf("") }
    val tab = vm.authTab

    AuthCard {
        AuthHeader("填写服务端地址，登录或注册")

        OutlinedTextField(
            value = base,
            onValueChange = { base = it },
            label = { Text("服务端地址") },
            placeholder = { Text("https://chat.example.com") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri)
        )
        Spacer(Modifier.height(16.dp))

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .clip(RoundedCornerShape(12.dp))
                .background(Color(0xFFF7F8FC))
                .padding(4.dp)
        ) {
            Button(
                onClick = { vm.authTab = 0 },
                modifier = Modifier.weight(1f),
                shape = RoundedCornerShape(9.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (tab == 0) Color.White else Color.Transparent,
                    contentColor = if (tab == 0) Brand else Color.Gray
                ),
                elevation = null
            ) { Text("登录") }
            Button(
                onClick = { vm.authTab = 1 },
                modifier = Modifier.weight(1f),
                shape = RoundedCornerShape(9.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (tab == 1) Color.White else Color.Transparent,
                    contentColor = if (tab == 1) Brand else Color.Gray
                ),
                elevation = null
            ) { Text("注册") }
        }
        Spacer(Modifier.height(16.dp))

        OutlinedTextField(
            value = user,
            onValueChange = { user = it },
            label = { Text("用户名") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp)
        )
        Spacer(Modifier.height(12.dp))
        OutlinedTextField(
            value = pass,
            onValueChange = { pass = it },
            label = { Text("密码") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp)
        )

        if (tab == 1) {
            Spacer(Modifier.height(12.dp))
            OutlinedTextField(
                value = pass2,
                onValueChange = { pass2 = it },
                label = { Text("确认密码") },
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp)
            )
            Spacer(Modifier.height(12.dp))
            OutlinedTextField(
                value = code,
                onValueChange = { code = it },
                label = { Text("注册邀请码") },
                placeholder = { Text("由管理员在 /admin 生成") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(12.dp)
            )
        }

        Spacer(Modifier.height(18.dp))
        Button(
            onClick = {
                if (tab == 0) {
                    vm.login(base, user.trim(), pass)
                } else if (pass != pass2) {
                    vm.showToast("两次输入的密码不一致")
                } else {
                    vm.register(base, user.trim(), pass, code.trim())
                }
            },
            enabled = !vm.busy,
            modifier = Modifier.fillMaxWidth().height(48.dp),
            shape = RoundedCornerShape(12.dp),
            colors = ButtonDefaults.buttonColors(containerColor = Brand)
        ) {
            Text(if (tab == 0) "登录" else "注册", fontSize = 15.sp)
        }
        Spacer(Modifier.height(10.dp))
        Text(
            "域名证书由反向代理提供，客户端无需任何证书配置",
            fontSize = 11.sp,
            color = Color.Gray,
            textAlign = TextAlign.Center,
            modifier = Modifier.fillMaxWidth()
        )
        Text(
            "每个账号都有用户名和密码，聊天界面会显示用户名",
            fontSize = 11.sp,
            color = Color.Gray,
            textAlign = TextAlign.Center,
            modifier = Modifier.fillMaxWidth()
        )
    }
}

@Composable
fun ChangePasswordScreen(vm: ChatViewModel) {
    var old by remember { mutableStateOf("") }
    var newPass by remember { mutableStateOf("") }
    var newPass2 by remember { mutableStateOf("") }

    AuthCard {
        AuthHeader("首次登录必须修改初始随机密码")
        OutlinedTextField(
            value = old,
            onValueChange = { old = it },
            label = { Text("原密码") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp)
        )
        Spacer(Modifier.height(12.dp))
        OutlinedTextField(
            value = newPass,
            onValueChange = { newPass = it },
            label = { Text("新密码（至少 8 位）") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp)
        )
        Spacer(Modifier.height(12.dp))
        OutlinedTextField(
            value = newPass2,
            onValueChange = { newPass2 = it },
            label = { Text("确认新密码") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(12.dp)
        )
        Spacer(Modifier.height(18.dp))
        Button(
            onClick = {
                if (newPass != newPass2) vm.showToast("两次输入的新密码不一致")
                else vm.changePassword(old, newPass)
            },
            enabled = !vm.busy,
            modifier = Modifier.fillMaxWidth().height(48.dp),
            shape = RoundedCornerShape(12.dp),
            colors = ButtonDefaults.buttonColors(containerColor = Brand)
        ) { Text("保存并进入", fontSize = 15.sp) }
        Spacer(Modifier.height(8.dp))
        TextButton(onClick = { vm.logout() }, modifier = Modifier.fillMaxWidth()) {
            Text("退出登录", fontSize = 12.sp, color = Color.Gray)
        }
    }
}
