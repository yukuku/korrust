package roots
inline fun describeArbitrary(v: Any): String = v.toString()
inline fun fingerprintArbitrary(v: Any): Int = v.hashCode()
inline fun equivalentArbitrary(v: Any, w: Any?): Boolean = v.equals(w)
inline fun readInherited(v: Leaf): Int = v.token
inline fun readEpoch(v: Leaf): Int = Leaf.epoch
inline fun <reified T> makeVacancies(n: Int): Array<T?> = arrayOfNulls<T>(n)
inline fun <reified T> gatherItems(values: Collection<T>): Array<T> = values.toTypedArray()

inline fun hiddenBase(v: Visible): Int = v.marker + v.payload()
inline fun <reified T> nestedVacancies(n: Int): Array<Array<T>?> = arrayOfNulls<Array<T>>(n)
