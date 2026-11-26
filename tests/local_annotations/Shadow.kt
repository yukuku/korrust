package localannotations
import localannotationlib.Suppress
fun shadowed(): Int {
    @Suppress(7)
    val value = 12
    return value
}
