package next.probes

object Scratch {
    private val buffer = ThreadLocal.withInitial { ByteArray(1) }
    @JvmStatic
    fun same(): Boolean = buffer == buffer
    @JvmStatic
    fun thisSame(): Boolean = this == this
}
