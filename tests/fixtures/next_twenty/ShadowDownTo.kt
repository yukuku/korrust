package next.shadow
import next.library.downTo
fun shadowDownTo(): Int { var total = 0; for (i in 1 downTo 3) total += i; return total }
