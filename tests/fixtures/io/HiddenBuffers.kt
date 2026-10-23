package korrust.probes.io

internal object HiddenBuffers {
    private val state = ThreadLocal.withInitial { CharArray(3) }
    @JvmStatic
    fun size(): Int {
        val value = state.get()!!
        return value.size
    }
}
