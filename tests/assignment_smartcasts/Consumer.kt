import assign.Lookup
fun consume(): Boolean {
    val lookup = Lookup()
    return literal() == 3 && generic(lookup, 1) == 4 && readonlyCapture(lookup, 1) == 5 && platformRead(lookup, 0) == null && branches(false) == 4 && readonlyLoop(2) == 9
}
