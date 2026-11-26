package operandfunctions
import kotlinx.coroutines.runBlocking
import receiverlibrary.checkpoint
object Events { var trace: Int = 0; fun add(value: Int): Int { trace = trace * 10 + value; return value } }
class Receiver { fun accept(a: Int, b: Int, c: Int): Int { Events.add(5); return a + b + c } }
fun receiver(): Receiver { Events.add(1); return Receiver() }
suspend fun delayed(): Int { Events.add(3); val value = checkpoint(); Events.add(4); return value }
fun ordered(): Int = runBlocking { receiver().accept(Events.add(2), delayed(), Events.add(6)) }
suspend fun combine(a: Int, b: Int): Int = a + b
fun nested(): Int = runBlocking { combine(delayed(), delayed()) }
fun callback(): (Int, Int) -> Int { Events.add(1); return { a, b -> Events.add(5); a + b } }
fun variable(): Int = runBlocking { val fn = callback(); fn(Events.add(2), delayed()) }
fun regular(a: Int, b: Int): Int { Events.add(5); return a + b }
fun direct(): Int = runBlocking { regular(Events.add(2), delayed()) }
fun fail(): Int { Events.add(2); throw IllegalStateException("operand") }
fun beforeFailure(): Int = runBlocking { regular(fail(), delayed()) }
fun afterFailure(): Int = runBlocking { regular(delayed(), fail()) }
