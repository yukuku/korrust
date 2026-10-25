package next.shadow
import next.library.step
fun shadowStep(): Int { var total = 0; for (i in 1..3 step 2) total += i; return total }
