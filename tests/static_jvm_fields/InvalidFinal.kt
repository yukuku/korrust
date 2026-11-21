class Bad { companion object { @JvmField val value: Int = 1 } }
fun bad() { Bad.value = 2 }
