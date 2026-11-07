# R8 full mode 下的最小保留规则；OkHttp / Compose 自带 consumer rules
-keepattributes *Annotation*, Signature, InnerClasses, EnclosingMethod, Exceptions
-dontwarn kotlinx.coroutines.**
-dontwarn okhttp3.internal.platform.**
-dontwarn org.conscrypt.**
-keepclassmembers class kotlin.Metadata { *; }
