package consumer

import enums.polymorphism.*

fun main() {
    check(Task.LEFT.run(6) == "L27")
    check(Task.LEFT.caption == "LEFT:4!")
    check(Task.entries[1] == Task.RIGHT)
    check(Task.values()[2] == Task.EMPTY)
    check(Task.valueOf("RIGHT") == Task.RIGHT)
    check(AbstractAction.A.score() == 8.5)
    check(PerEntry.B.caption == "B")
    check(DefaultOnly.B.following == 12)
    val action: Action = Task.LEFT
    check(action.run(2) == "L23")
    println("OK")
}
