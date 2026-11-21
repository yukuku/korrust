package receiveroracle
object Receiver { @JvmField var value: Int = 0 }
private var trace: String = ""
fun factory(): Receiver { trace += "R"; return Receiver }
fun rhs(): Int { trace += "V"; return 14 }
fun observation(): String {
    trace = ""
    factory().value = rhs()
    val result = factory().value
    return trace + result
}
