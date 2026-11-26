package inherited.library
open class CompiledMid: JavaTypes.Mid()
open class KotlinBase {
    private class KHidden
    class Nested(val value: String)
    protected class KProtected { fun tag(): String = "k-protected" }
}
open class OpaqueBase {
    @JvmInline value class Opaque<T>(val value: T)
}

abstract class SuspendBase: JavaTypes.Base() {
    abstract suspend fun execute(): Result
}
