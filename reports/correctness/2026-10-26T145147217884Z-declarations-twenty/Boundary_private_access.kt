class Secret { private fun value(): Int = 1 }
fun bad(): Int = Secret().value()
