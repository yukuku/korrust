import yuku.alkitab.datatransfer.model.Gid
import yuku.alkitab.datatransfer.model.Rpp
fun main() {
    val gid = Gid("abc")
    check(gid.value == "abc")
    check(gid.toString() == "abc")
    val rpp = Rpp(gid, 7L, listOf(1, 2))
    check(rpp.gid == gid)
    check(rpp.component1() == gid)
    check(rpp.copy(startTime = 8L).startTime == 8L)
    check(rpp == Rpp(Gid("abc"), 7L, listOf(1, 2)))
    check(rpp.toString() == "Rpp(gid=abc, startTime=7, done=[1, 2])")
    println("OK")
}
