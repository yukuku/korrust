package compatibility.declarations.consumer
import compatibility.declarations.*
import yuku.alkitab.datatransfer.process.LogInterface
fun main() {
    var lines = ""
    val target = LogInterface { line -> lines += line + ";" }
    val log = target.logger("K")
    log("first")
    log("second")
    check(lines == "[K] first;[K] second;")
    check(appCapturedLog("P") == "[P] alpha;[P] beta;")
    println("OK")
}
