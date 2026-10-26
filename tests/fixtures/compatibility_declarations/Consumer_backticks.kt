package compatibility.declarations.consumer
import compatibility.declarations.*
fun main() {
    check(escapedCall(10) == 11)
    check(DescriptiveTest().`plain helper`(4) == 6)
    println("OK")
}
