package compatibility.external
import compatibility.visibility.InternalClass

fun illegalClassInternal(): Int = InternalClass().internalValue(2)
