package com.chatware.app.ui

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.webkit.MimeTypeMap
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.FileProvider
import com.chatware.app.model.ChatMessage
import com.chatware.app.model.FileMeta
import com.chatware.app.net.displayName
import com.chatware.app.ui.theme.Brand
import com.chatware.app.ui.theme.ChatBackground
import com.chatware.app.ui.theme.WechatGreen
import kotlinx.coroutines.launch
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private fun fmtSize(size: Long): String = when {
    size < 1024 -> "$size B"
    size < 1048576 -> String.format(Locale.CHINA, "%.1f KB", size / 1024.0)
    else -> String.format(Locale.CHINA, "%.1f MB", size / 1048576.0)
}

private fun fmtStamp(ts: Long): String =
    SimpleDateFormat("MM/dd HH:mm", Locale.CHINA).format(Date(ts))

private fun openDownloaded(context: Context, file: File) {
    val uri = FileProvider.getUriForFile(context, "${context.packageName}.fileprovider", file)
    val mime = MimeTypeMap.getSingleton()
        .getMimeTypeFromExtension(file.extension.lowercase(Locale.ROOT)) ?: "*/*"
    val intent = Intent(Intent.ACTION_VIEW).apply {
        setDataAndType(uri, mime)
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    }
    try {
        context.startActivity(intent)
    } catch (e: Exception) {
        Toast.makeText(context, "已保存到 ${file.absolutePath}", Toast.LENGTH_LONG).show()
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ChatScreen(vm: ChatViewModel) {
    val context = LocalContext.current
    val clipboard = LocalClipboardManager.current
    val listState = rememberLazyListState()
    val scope = rememberCoroutineScope()
    var input by remember { mutableStateOf("") }
    var showAccount by remember { mutableStateOf(false) }

    val picker = rememberLauncherForActivityResult(ActivityResultContracts.GetContent()) { uri: Uri? ->
        if (uri != null) {
            vm.uploadFile(context, uri, displayName(context, uri))
        }
    }

    fun onFileClick(meta: FileMeta) {
        scope.launch {
            try {
                val file = vm.downloadFile(context, meta.id, meta.name)
                Toast.makeText(context, "已下载：${file.name}", Toast.LENGTH_SHORT).show()
                openDownloaded(context, file)
            } catch (e: Exception) {
                Toast.makeText(context, e.message ?: "下载失败", Toast.LENGTH_SHORT).show()
            }
        }
    }

    LaunchedEffect(vm.messages.lastOrNull()?.id) {
        if (vm.messages.isNotEmpty()) {
            listState.animateScrollToItem(vm.messages.size - 1)
        }
    }

    Scaffold(
        containerColor = Color.White,
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text("Chatware", fontSize = 17.sp, fontWeight = FontWeight.Bold)
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Box(
                                Modifier
                                    .size(7.dp)
                                    .clip(CircleShape)
                                    .background(if (vm.connected) Color(0xFF10B981) else Color.Gray)
                            )
                            Spacer(Modifier.width(5.dp))
                            Text(
                                if (vm.connected) "公共大厅 · 已连接" else "连接中…",
                                fontSize = 11.sp,
                                color = Color.Gray
                            )
                        }
                    }
                },
                actions = {
                    TextButton(onClick = { showAccount = true }) {
                        Text("账号", fontSize = 12.sp, color = Brand)
                    }
                    TextButton(onClick = {
                        clipboard.setText(AnnotatedString(vm.serverUrl))
                        Toast.makeText(context, "已复制服务端地址", Toast.LENGTH_SHORT).show()
                    }) { Text("复制地址", fontSize = 12.sp, color = Brand) }
                    TextButton(onClick = { vm.gotoAuth() }) {
                        Text("切换", fontSize = 12.sp, color = Color.Gray)
                    }
                    TextButton(onClick = { vm.logout() }) {
                        Text("退出", fontSize = 12.sp, color = Color.Gray)
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(containerColor = Color.White)
            )
        },
        bottomBar = {
            Composer(
                value = input,
                onChange = { input = it },
                onSend = {
                    vm.send(input)
                    input = ""
                },
                onAttach = { picker.launch("*/*") }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .background(ChatBackground)
        ) {
            if (vm.hasMore) {
                TextButton(
                    onClick = { vm.loadHistory(reset = false) },
                    modifier = Modifier.align(Alignment.CenterHorizontally)
                ) {
                    Text("加载更早的消息", fontSize = 12.sp, color = Color.Gray)
                }
            }
            LazyColumn(
                state = listState,
                modifier = Modifier.fillMaxWidth().weight(1f),
                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp)
            ) {
                items(vm.messages, key = { it.id }) { message ->
                    MessageRow(
                        msg = message,
                        myName = vm.myName,
                        onDownload = { meta -> onFileClick(meta) }
                    )
                }
            }
        }
    }

    if (showAccount) {
        AccountDialog(vm = vm, onDismiss = { showAccount = false })
    }
}

/** 账号设置：改用户名 / 密码。成功后服务端吊销会话，本页随 Screen 切换自动退出。 */
@Composable
private fun AccountDialog(vm: ChatViewModel, onDismiss: () -> Unit) {
    var user by remember { mutableStateOf(vm.myName) }
    var current by remember { mutableStateOf("") }
    var newPass by remember { mutableStateOf("") }
    var newPass2 by remember { mutableStateOf("") }
    val scroll = rememberScrollState()

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("账号设置", fontWeight = FontWeight.Bold) },
        text = {
            Column(Modifier.verticalScroll(scroll)) {
                Text(
                    "可修改用户名或密码；保存后需要用新的账号信息重新登录。",
                    fontSize = 12.sp,
                    color = Color.Gray
                )
                Spacer(Modifier.height(12.dp))
                OutlinedTextField(
                    value = user,
                    onValueChange = { user = it },
                    label = { Text("用户名") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp)
                )
                Spacer(Modifier.height(10.dp))
                OutlinedTextField(
                    value = current,
                    onValueChange = { current = it },
                    label = { Text("当前密码") },
                    singleLine = true,
                    visualTransformation = PasswordVisualTransformation(),
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp)
                )
                Spacer(Modifier.height(10.dp))
                OutlinedTextField(
                    value = newPass,
                    onValueChange = { newPass = it },
                    label = { Text("新密码（留空不改）") },
                    singleLine = true,
                    visualTransformation = PasswordVisualTransformation(),
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp)
                )
                Spacer(Modifier.height(10.dp))
                OutlinedTextField(
                    value = newPass2,
                    onValueChange = { newPass2 = it },
                    label = { Text("确认新密码") },
                    singleLine = true,
                    visualTransformation = PasswordVisualTransformation(),
                    modifier = Modifier.fillMaxWidth(),
                    shape = RoundedCornerShape(12.dp)
                )
            }
        },
        confirmButton = {
            Button(
                onClick = {
                    if (newPass != newPass2) {
                        vm.showToast("两次输入的新密码不一致")
                    } else {
                        vm.updateAccount(current, user.trim(), newPass)
                    }
                },
                enabled = !vm.busy,
                shape = RoundedCornerShape(10.dp),
                colors = ButtonDefaults.buttonColors(containerColor = Brand)
            ) { Text("保存") }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("取消", color = Color.Gray) }
        }
    )
}

@Composable
private fun Composer(
    value: String,
    onChange: (String) -> Unit,
    onSend: () -> Unit,
    onAttach: () -> Unit
) {
    Surface(color = Color(0xFFF7F7F7), shadowElevation = 2.dp) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 10.dp),
            verticalAlignment = Alignment.Bottom
        ) {
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(Color.White)
                    .clickable(onClick = onAttach),
                contentAlignment = Alignment.Center
            ) {
                Text("+", fontSize = 22.sp, color = Color(0xFF6B7280))
            }
            Spacer(Modifier.width(8.dp))
            OutlinedTextField(
                value = value,
                onValueChange = onChange,
                modifier = Modifier.weight(1f).heightIn(min = 44.dp, max = 120.dp),
                placeholder = { Text("输入消息", fontSize = 14.sp) },
                shape = RoundedCornerShape(12.dp),
                maxLines = 4,
                colors = OutlinedTextFieldDefaults.colors(
                    focusedContainerColor = Color.White,
                    unfocusedContainerColor = Color.White,
                    focusedBorderColor = Brand,
                    unfocusedBorderColor = Color(0xFFE3E5EA)
                ),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
                keyboardActions = KeyboardActions(onSend = { onSend() })
            )
            Spacer(Modifier.width(8.dp))
            Button(
                onClick = onSend,
                modifier = Modifier.height(44.dp),
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(containerColor = Brand)
            ) { Text("发送") }
        }
    }
}

@Composable
private fun MessageRow(
    msg: ChatMessage,
    myName: String,
    onDownload: (FileMeta) -> Unit
) {
    val isMe = msg.username == myName
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 7.dp),
        horizontalArrangement = if (isMe) Arrangement.End else Arrangement.Start,
        verticalAlignment = Alignment.Top
    ) {
        if (!isMe) {
            Avatar(msg.username)
            Spacer(Modifier.width(8.dp))
        }
        Column(
            modifier = Modifier.weight(1f, fill = false),
            horizontalAlignment = if (isMe) Alignment.End else Alignment.Start
        ) {
            Text(
                "${msg.username} · ${fmtStamp(msg.ts)}",
                fontSize = 11.sp,
                color = Color(0xFF9299A8)
            )
            Spacer(Modifier.height(4.dp))
            Box {
                // 气泡小尖角
                Box(
                    modifier = Modifier
                        .size(10.dp)
                        .align(if (isMe) Alignment.CenterEnd else Alignment.CenterStart)
                        .offset(x = if (isMe) 4.dp else (-4).dp, y = 4.dp)
                        .graphicsLayer(rotationZ = 45f)
                        .background(if (isMe) WechatGreen else Color.White)
                )
                Box(
                    modifier = Modifier
                        .widthIn(max = 270.dp)
                        .clip(RoundedCornerShape(8.dp))
                        .background(if (isMe) WechatGreen else Color.White)
                        .padding(10.dp)
                ) {
                    val file = msg.file
                    if (msg.kind == "file" && file != null) {
                        FileBubble(file) { onDownload(it) }
                    } else {
                        Text(
                            msg.text,
                            fontSize = 15.sp,
                            lineHeight = 21.sp,
                            color = Color(0xFF1F2430)
                        )
                    }
                }
            }
        }
        if (isMe) {
            Spacer(Modifier.width(8.dp))
            Avatar(msg.username)
        }
    }
}

@Composable
private fun Avatar(name: String) {
    val hue = ((name.hashCode().toLong() and 0x7fffffffL) % 360).toFloat()
    Box(
        modifier = Modifier
            .size(40.dp)
            .clip(RoundedCornerShape(9.dp))
            .background(Color.hsl(hue, 0.62f, 0.58f)),
        contentAlignment = Alignment.Center
    ) {
        Text(name.take(1), color = Color.White, fontSize = 16.sp, fontWeight = FontWeight.Bold)
    }
}

@Composable
private fun FileBubble(meta: FileMeta, onClick: (FileMeta) -> Unit) {
    Row(
        modifier = Modifier.widthIn(min = 200.dp).clickable { onClick(meta) },
        verticalAlignment = Alignment.CenterVertically
    ) {
        Box(
            modifier = Modifier
                .size(36.dp)
                .clip(RoundedCornerShape(9.dp))
                .background(Color(0x1F5B6CFF)),
            contentAlignment = Alignment.Center
        ) {
            Text("▤", color = Brand, fontSize = 16.sp)
        }
        Spacer(Modifier.width(10.dp))
        Column(Modifier.widthIn(max = 190.dp)) {
            Text(
                meta.name,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                fontSize = 13.sp,
                fontWeight = FontWeight.SemiBold,
                color = Color(0xFF1F2430)
            )
            Text(
                "${fmtSize(meta.size)} · ${meta.uploader} · 点击下载",
                fontSize = 11.sp,
                color = Color(0xFF8A91A3)
            )
        }
    }
}
