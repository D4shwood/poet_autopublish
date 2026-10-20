package com.d4shwood.poet

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import com.d4shwood.poet.ui.PoetApp
import com.d4shwood.poet.ui.PoetTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            PoetTheme { PoetApp() }
        }
    }
}
