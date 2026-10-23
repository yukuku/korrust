package korrust.probes.io

object LazyScratch {
    private val failedBytes = ThreadLocal.withInitial { ByteArray(-1) }
    private val failedChars = ThreadLocal.withInitial { CharArray(-1) }
    @JvmStatic
    fun touchBytes(): ByteArray = failedBytes.get()!!
    @JvmStatic
    fun touchChars(): CharArray = failedChars.get()!!
    @JvmStatic
    fun constant(): Int = 17
}
