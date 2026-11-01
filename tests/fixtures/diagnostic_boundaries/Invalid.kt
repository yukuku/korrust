import boundarylib.Crate
import boundarylib.unpack
fun wrong(value: Crate<String>): Int = value.unpack("tag")
