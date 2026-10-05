# JNA finds its classes and the generated bindings' structures by reflection, so the code
# shrinker must keep them.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class io.github.pheasakhmer.keyboard.core.** { *; }
-dontwarn java.awt.**
