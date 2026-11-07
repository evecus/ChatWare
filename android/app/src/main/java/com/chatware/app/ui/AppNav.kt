package com.chatware.app.ui

import android.widget.Toast
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.viewmodel.compose.viewModel
import com.chatware.app.data.Prefs

@Composable
fun AppRoot(prefs: Prefs) {
    val vm: ChatViewModel = viewModel(factory = ChatViewModel.factory(prefs))
    val context = LocalContext.current

    LaunchedEffect(vm.toast) {
        vm.toast?.let { message ->
            Toast.makeText(context, message, Toast.LENGTH_SHORT).show()
            vm.showToast(null)
        }
    }

    when (vm.screen) {
        Screen.Server -> ServerScreen(prefs, vm)
        Screen.Login -> LoginScreen(vm)
        Screen.ChangePassword -> ChangePasswordScreen(vm)
        Screen.Chat -> ChatScreen(vm)
    }
}
