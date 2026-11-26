package inherited.consumer

import inherited.library.JavaTypes.Base
import inherited.library.JavaTypes.Mid
import inherited.library.JavaTypes.Near
import inherited.library.JavaTypes.PrivateHider
import inherited.library.CompiledMid
import inherited.library.KotlinBase
import inherited.library.OpaqueBase
import inherited.library.SuspendBase

class Direct: Mid() {
    fun make(): Result = Result.make("direct")
    fun bodyOnly() = Result("body")
    fun token() = Token("token")
    fun protectedName(): String = Protected.tag()
    fun localShadow(): String { val Result: String = "local"; return Result }
}
open class SourceMid: Base()
class SourceEnd: SourceMid() { fun make(): Result = Result("source") }
class CompiledEnd: CompiledMid() { fun make(): Result = Result("compiled") }
class Closest: Near() { fun make(): Result = Result("closest") }
class HiddenSkipped: PrivateHider() { fun make(): Result = Result("visible-ancestor") }
class KotlinEnd: KotlinBase() {
    fun make(): Nested = Nested("kotlin")
    fun protectedName(): String = KProtected().tag()
}
class Own: Base() {
    class Result(val value: String)
    fun make(): Result = Result("own")
}
class OwnOpaque: OpaqueBase() {
    class Opaque(val value: String)
    fun make(): Opaque = Opaque("shadow-opaque")
}
class TypeParameter<Result>: Base() { fun identity(value: Result): Result = value }
class FunctionParameter: Base() { fun <Result> identity(value: Result): Result = value }
class BuiltinShadow: Base() { fun name(): String = ArrayList.tag() }
fun ordinaryList(): java.util.ArrayList<String> = ArrayList(listOf("ordinary"))

class SuspendEnd: SuspendBase() {
    override suspend fun execute(): Result = Result.make("suspend")
}
