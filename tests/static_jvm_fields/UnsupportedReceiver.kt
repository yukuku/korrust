object Receiver { @JvmField var value: Int = 0 }
fun factory(): Receiver = Receiver
fun access(): Int { factory().value = 2; return factory().value }
