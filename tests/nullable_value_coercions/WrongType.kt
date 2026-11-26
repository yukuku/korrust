package nullablevalues.consumer
import nullablevalues.library.OtherMode
import nullablevalues.library.inspectMode
fun invalid(): Int = inspectMode(OtherMode(1))
