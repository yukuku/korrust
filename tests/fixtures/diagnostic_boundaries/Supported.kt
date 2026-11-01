import boundarylib.Crate
import boundarylib.Ticket
import boundarylib.unpack
import boundarylib.doubleIt
fun reveal(value: Crate<String>): String = value.unpack("tag")
fun ticket(value: Int): Int = Ticket(value).doubleIt()
