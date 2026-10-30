import kotlin.jvm.internal.Intrinsics
inline fun bad(block: () -> Int) { Intrinsics.checkNotNullParameter(block, "block") }
