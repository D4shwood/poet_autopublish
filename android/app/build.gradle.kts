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
        // A release passes these (-PversionCode=… -PversionName=…); each must grow
        // for an update to install over the version before.
        versionCode = (findProperty("versionCode") as String?)?.toInt() ?: 1
        versionName = (findProperty("versionName") as String?) ?: "0.1.0-dev"
        // Only the architectures build-rust.sh builds the core for.
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }

    // The release key, when given: POET_KEYSTORE (a file) and POET_KEYSTORE_PASSWORD.
    // Every release must carry the same key, or phones refuse the update.
    val keystore = System.getenv("POET_KEYSTORE")?.let(::file)?.takeIf { it.exists() }
    signingConfigs {
        if (keystore != null) {
            create("release") {
                storeFile = keystore
                storePassword = System.getenv("POET_KEYSTORE_PASSWORD")
                keyAlias = "poet"
                keyPassword = System.getenv("POET_KEYSTORE_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // Without the release key (a local build), the debug key.
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
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
    implementation("androidx.work:work-runtime-ktx:2.10.0")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    // UniFFI's Kotlin bindings call the Rust library through JNA.
    implementation("net.java.dev.jna:jna:5.15.0@aar")
    debugImplementation("androidx.compose.ui:ui-tooling")
    testImplementation("junit:junit:4.13.2")
}

// The Rust core: native libraries and Kotlin bindings, rebuilt before each build.
val buildRust by tasks.registering(Exec::class) {
    workingDir = rootProject.projectDir
    commandLine("sh", "build-rust.sh", "release")
}
tasks.named("preBuild") { dependsOn(buildRust) }
