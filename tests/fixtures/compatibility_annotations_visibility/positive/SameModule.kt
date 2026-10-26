package compatibility.visibility

fun sameModule(): Int = moduleOnly(2) + InternalObject.decorate(3)
