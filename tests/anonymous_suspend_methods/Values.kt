package anonymousfunctions
import anonymouslibrary.Service
import anonymouslibrary.Generic
import anonymouslibrary.Base
import anonymouslibrary.SuspendSam
import anonymouslibrary.GenericSam
import anonymouslibrary.Names
import anonymouslibrary.UnitSam
import anonymouslibrary.Wide
import receiverlibrary.checkpoint
import kotlinx.coroutines.runBlocking
fun make(seed: Int): Service {
    var total = seed
    return object : Service {
        var calls = 0
        override fun ready(): Int = total + calls
        override suspend fun first(input: Int): Int {
            val original = this
            calls++
            val value = checkpoint()
            if (original !== this) throw IllegalStateException("receiver")
            total += input + value
            return total + calls
        }
        override suspend fun second(input: Int): Int {
            val first = checkpoint()
            val second = checkpoint()
            return total + calls + input + first + second
        }
        override suspend fun text(input: String): String? {
            if (input == "null") return null
            val value = checkpoint()
            return input + value
        }
        override suspend fun finish(input: Int) {
            val value = checkpoint()
            calls += input + value
        }
    }
}
fun generic(): Generic<String> = object : Generic<String> {
    override suspend fun echo(value: String): String {
        val result = checkpoint()
        return value + result
    }
}
fun base(seed: Int): Base = object : Base(seed) {
    override suspend fun get(input: Int): Int {
        val value = checkpoint()
        return bias + input + value
    }
}
fun first(value: Service, input: Int): Int = runBlocking { value.first(input) }
fun second(value: Service, input: Int): Int = runBlocking { value.second(input) }
fun text(value: Service, input: String): String? = runBlocking { value.text(input) }
fun finish(value: Service, input: Int): Int = runBlocking { value.finish(input); value.ready() }
fun echo(value: Generic<String>, input: String): String = runBlocking { value.echo(input) }
fun get(value: Base, input: Int): Int = runBlocking { value.get(input) }
fun genericInt(): Generic<Int> = object : Generic<Int> {
    override suspend fun echo(value: Int): Int {
        val result = checkpoint()
        return value + result
    }
}
fun echoInt(value: Generic<Int>, input: Int): Int = runBlocking { value.echo(input) }
fun sam(seed: Int): SuspendSam = SuspendSam { input ->
    val value = checkpoint()
    seed + input + value
}
fun genericSam(): GenericSam<String> = GenericSam<String> { input ->
    val value = checkpoint()
    input + value
}
fun callSam(value: SuspendSam, input: Int): Int = runBlocking { value.apply(input) }
fun callGenericSam(value: GenericSam<String>, input: String): String = runBlocking { value.apply(input) }
fun narrow(): String {
    val value = object : Names() { override fun name() = "ok" }
    val result: String = value.name()
    return result
}
fun declared(): Names = object : Names() { override fun name(): String = "declared" }
fun unitSam(): UnitSam = UnitSam { checkpoint() }
fun callUnitSam(value: UnitSam): Int = runBlocking { value.run(); 7 }
fun wide(): Wide = object : Wide {
    override suspend fun run(value: Long, ratio: Double): Long {
        val kept = ratio
        val result = checkpoint()
        return value + result + kept.toLong()
    }
}
fun callWide(value: Wide): Long = runBlocking { value.run(7L, 2.0) }
