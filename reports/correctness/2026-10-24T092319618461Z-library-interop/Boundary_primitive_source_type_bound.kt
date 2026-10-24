fun <T : Long> longIdentity(value: T): T = value
fun boundary(): Long = longIdentity<Long>(4L)
