package preexisting
private var effects: Int = 0
private fun effect(): String { effects += 1; return "unused" }
fun observed(): Int {
    effects = 0
    try { Api.broken().indexOf(effect()) } catch (failure: NullPointerException) { return effects }
    return -1
}
