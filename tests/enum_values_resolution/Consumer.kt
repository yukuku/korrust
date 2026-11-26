package usage
import fixture.Holder
import fixture.Holder.Status
import fixture.Ordinary
fun qualified(): Array<Status> = Holder.Status.values()
fun simple(): Array<Status> = Status.values()
fun overload(): Int = Status.values(5)
fun shadowed(Status: Ordinary): Int = Status.values()
