plugins {
    alias(libs.plugins.android.application)
}

/** Runs cargo in the repository root and writes into [outputDirectory], which AGP adds
 *  to a variant's sources. */
abstract class Cargo : Exec() {
    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty
}

// The Rust workspace is the repository root, one level up.
val repository: File = rootDir.parentFile
val abis = listOf("arm64-v8a", "armeabi-v7a", "x86_64")
val minimumSdk = 24
val ndkRelease = "29.0.14206865"

// Gradle started from Android Studio does not see the shell's PATH, so look for cargo
// (and cargo-ndk) where rustup and Homebrew put them too.
val searchPath = listOf(
    "${System.getProperty("user.home")}/.cargo/bin",
    "/opt/homebrew/opt/rustup/bin",
    System.getenv("PATH").orEmpty(),
).joinToString(File.pathSeparator)
val cargo: String = searchPath.split(File.pathSeparator)
    .map { File(it, "cargo") }
    .firstOrNull { it.canExecute() }?.path ?: "cargo"

val rustSources = files(
    repository.resolve("Cargo.toml"),
    repository.resolve("Cargo.lock"),
    fileTree(repository.resolve("core/src")),
    fileTree(repository.resolve("ffi")),
)

// The full lexicon if it was exported (see data/README.md), else the sample in the repository.
// -Pkhmer.data=DIR picks another export.
val keyboardData: File = providers.gradleProperty("khmer.data").orNull?.let(repository::resolve)
    ?: repository.resolve("data/build").takeIf { it.isDirectory }
    ?: repository.resolve("data/sample")

android {
    namespace = "io.github.pheasakhmer.keyboard"
    compileSdk = 37
    ndkVersion = ndkRelease

    defaultConfig {
        applicationId = "io.github.pheasakhmer.keyboard"
        minSdk = minimumSdk
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += abis }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // The data file is read whole at start-up; storing it uncompressed keeps that fast.
    androidResources { noCompress += "kbd" }
}

androidComponents {
    onVariants { variant ->
        val name = variant.name.replaceFirstChar(Char::uppercase)
        val ndk = sdkComponents.sdkDirectory.get().dir("ndk/$ndkRelease").asFile.path

        val targets = abis.flatMap { listOf("-t", it) }
        val platform = "$minimumSdk"
        val dataDirectory = keyboardData.path

        val core = tasks.register<Cargo>("buildCore$name") {
            description = "Builds the Rust core for each Android ABI with cargo-ndk."
            val out = outputDirectory
            workingDir(repository)
            environment("PATH", searchPath)
            environment("ANDROID_NDK_HOME", ndk)
            inputs.files(rustSources)
            executable(cargo)
            argumentProviders.add {
                listOf("ndk") + targets + listOf(
                    "--platform", platform, "-o", out.get().asFile.path,
                    "build", "--release", "-p", "khmer-ffi",
                )
            }
        }

        val bindings = tasks.register<Cargo>("generateCoreBindings$name") {
            description = "Generates the core's Kotlin bindings with UniFFI."
            val out = outputDirectory
            val library = core.flatMap { it.outputDirectory.file("arm64-v8a/libkhmer_ffi.so") }
            workingDir(repository)
            environment("PATH", searchPath)
            inputs.file(library)
            executable(cargo)
            argumentProviders.add {
                listOf(
                    "run", "--quiet", "-p", "khmer-ffi", "--features", "bindgen",
                    "--bin", "uniffi-bindgen", "--", "generate",
                    "--library", library.get().asFile.path,
                    "--language", "kotlin", "--no-format", "--out-dir", out.get().asFile.path,
                )
            }
        }

        val data = tasks.register<Cargo>("compileKeyboardData$name") {
            description = "Compiles the exported lexicon into the keyboard's data file."
            val out = outputDirectory
            workingDir(repository)
            environment("PATH", searchPath)
            inputs.dir(dataDirectory)
            executable(cargo)
            argumentProviders.add {
                listOf(
                    "run", "--quiet", "--release", "-p", "khmer-kbd", "--",
                    "compile", dataDirectory, out.get().file("khmer.kbd").asFile.path,
                )
            }
        }

        variant.sources.jniLibs?.addGeneratedSourceDirectory(core, Cargo::outputDirectory)
        variant.sources.kotlin?.addGeneratedSourceDirectory(bindings, Cargo::outputDirectory)
        variant.sources.assets?.addGeneratedSourceDirectory(data, Cargo::outputDirectory)
    }
}

dependencies {
    // The bindings call the core through JNA, which ships its own native library.
    implementation(libs.jna) { artifact { type = "aar" } }
    testImplementation(libs.junit)
}
