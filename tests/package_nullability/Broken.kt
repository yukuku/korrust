package defaults.consumer
private var effects: Int = 0
fun brokenRequired(): String = nonnull.Library.broken()
fun brokenNullable(): String? = nonnull.Library.broken()
fun brokenInferred(): Int { val value = nonnull.Library.broken(); return if (value == null) 1 else 2 }
fun brokenElvis(): String = nonnull.Library.broken() ?: "fallback"
fun brokenSafe(): Int? = nonnull.Library.broken()?.length
fun brokenDiscarded(): Int { nonnull.Library.broken(); return 3 }

private fun indexEffect(): Int { effects += 1; return 0 }
fun brokenArrayReceiver(): Int {
    effects = 0
    try { nonnull.Library.brokenArray()[indexEffect()] } catch (failure: NullPointerException) { return effects }
    return -1
}
fun requiredField(): String = nonnull.Library().brokenField
fun nullableFieldContext(): String? = nonnull.Library().brokenField

fun brokenNodeReceiver(): Int {
    effects = 0
    try { nonnull.Library.brokenNode().touch(indexEffect()) } catch (failure: NullPointerException) { return effects }
    return -1
}

fun brokenListReceiver(): Int {
    effects = 0
    try { nonnull.Library.brokenList().get(indexEffect()) } catch (failure: NullPointerException) { return effects }
    return -1
}

private fun String.sourceReceiver(value: Int): Int = value
fun brokenExtensionReceiver(): Int {
    effects = 0
    try { nonnull.Library.broken().sourceReceiver(indexEffect()) } catch (failure: NullPointerException) { return effects }
    return -1
}

private fun String?.nullableSourceReceiver(value: Int): Int = value
fun nullableExtensionReceiver(): Int {
    effects = 0
    nonnull.Library.broken().nullableSourceReceiver(indexEffect())
    return effects
}
