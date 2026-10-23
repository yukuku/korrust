package korrust.probes.io

object StaticReceiver {
    private fun ping(): Boolean { throw IllegalArgumentException("receiver-evaluated") }
    @JvmStatic
    fun value(): Int = 1
    @JvmStatic
    fun run(): Int = (if (ping()) { this } else { this }).value()
}
