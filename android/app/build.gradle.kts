plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.d4shwood.poet"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.d4shwood.poet"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        // Only the architectures build-rust.sh builds the core for.
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // Signed with the debug key until there is a real release key.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.12.01")
    implementation(composeBom)
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.documentfile:documentfile:1.0.1")
    // UniFFI's Kotlin bindings call the Rust library through JNA.
    implementation("net.java.dev.jna:jna:5.15.0@aar")
    debugImplementation("androidx.compose.ui:ui-tooling")
}

// The Rust core: native libraries and Kotlin bindings, rebuilt before each build.
val buildRust by tasks.registering(Exec::class) {
    workingDir = rootProject.projectDir
    commandLine("sh", "build-rust.sh", "release")
}
tasks.named("preBuild") { dependsOn(buildRust) }
