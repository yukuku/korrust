package korrust.probes.objects

internal object InternalObject {
    const val VALUE: Int = 37
    public const val PUBLIC: Int = 41
    const internal val INTERNAL = 43
    object Child { const val FLAG = true }
}
