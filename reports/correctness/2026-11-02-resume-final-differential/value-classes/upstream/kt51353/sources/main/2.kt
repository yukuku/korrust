
@JvmInline
value class Z(val value: String)

object O {
    val A = Z("OK")
    val B = A
}

fun box(): String = O.B.value
