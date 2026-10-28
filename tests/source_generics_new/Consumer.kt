import sourcegeneric.*
fun consume(): String {
    val box = Box<String>("O")
    box.value = box.identity("OK")
    val echo: Echo<String> = StringEcho()
    val numeric: Relay<Int> = IntEcho()
    val parent: Parent<String> = StringParent("start")
    val bounded: Bounded<Int> = Bounded<Int>(7)
    if (numeric.echo(8) != 9 || parent.original != "start" || parent.replace("O") != "OK" || bounded.value != 7) return "bad"
    val slot: Slot<Int> = IntSlot(4)
    slot.value = 5
    if (slot.value != 5) return "bad"
    return if (box.value == "OK") echo.echo("O") else "bad"
}
