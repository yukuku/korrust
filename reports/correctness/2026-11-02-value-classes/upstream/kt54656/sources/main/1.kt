@JvmInline
value class Email(val address: String)

fun box():String {
    J(Email("test"))
    return "OK"
}

