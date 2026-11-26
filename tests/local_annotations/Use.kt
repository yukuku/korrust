package localannotations
import kotlin.Suppress as Ignore
import localannotationlib.Mark
@Target(AnnotationTarget.LOCAL_VARIABLE)
@Retention(AnnotationRetention.SOURCE)
annotation class Own(val value: String)
fun local(): Int {
    @Ignore("UNCHECKED_CAST")
    val first = 3
    @Own("source")
    @Mark(4)
    var second = first + 1
    second += 2
    @kotlin.Suppress("RestrictedApi")
    return (@Mark(7) second)
}
fun expression(): Int = @Ignore("UNCHECKED_CAST") (5 + 6)
fun localFunction(): Int {
    @Ignore("UNCHECKED_CAST")
    fun nested(): Int = 8
    return nested()
}
fun arrayNames(): Int {
    @Ignore(names = ["UNCHECKED_CAST", "RestrictedApi"])
    val value = 9
    return value
}
