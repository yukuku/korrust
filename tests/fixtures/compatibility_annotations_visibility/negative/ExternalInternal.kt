package compatibility.external
import compatibility.visibility.InternalObject
import compatibility.visibility.moduleOnly
fun illegalInternal(): Int = moduleOnly(1) + InternalObject.decorate(2)
