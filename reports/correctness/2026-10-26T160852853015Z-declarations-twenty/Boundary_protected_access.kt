open class Secret { protected fun value(): Int = 1 }
fun bad(): Int = Secret().value()
