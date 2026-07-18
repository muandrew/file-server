# Bazel Kotlin & Android Project

A Bazel 8 project configured with Kotlin, Android development support, and Gazelle.

### Project Structure & Features

- **Bazel 8 Bzlmod Configuration**: `.bazelversion` set to `8.1.0` with standard Bzlmod declarations in `MODULE.bazel`.
- **Gazelle Target**: `BUILD.bazel` configured with `gazelle(name = "gazelle")`.
- **Kotlin JVM CLI App**:
  - Code: `apps/cli/src/main/kotlin/com/example/cli/Main.kt`
  - Target: `apps/cli/BUILD.bazel` (`kt_jvm_binary`)
  - Run via: `bazel run //apps/cli:cli`
- **Kotlin Android App**:
  - Activity: `apps/android/src/main/kotlin/com/example/android/MainActivity.kt`
  - Manifest & Resources: `apps/android/AndroidManifest.xml`, `apps/android/src/main/res/layout/activity_main.xml`
  - Target: `apps/android/BUILD.bazel` (`kt_android_library` + `android_binary`)
  - Build via: `bazel build //apps/android:app`

### Quick Commands

```bash
# Run Kotlin JVM CLI
bazel run //apps/cli:cli

# Build Android APK
bazel build //apps/android:app

# Run Gazelle
bazel run //:gazelle
```
