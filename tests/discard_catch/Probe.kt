import java.lang.IllegalStateException
import java.lang.IllegalArgumentException

fun probe(which: Int): Int {
    var done = 0
    val value = try {
        if (which == 1) throw IllegalArgumentException()
        if (which == 2) throw IllegalStateException()
        10
    } catch (_: IllegalArgumentException) {
        20
    } catch (_: IllegalStateException) {
        30
    } finally {
        done = 1
    }
    return value + done
}
