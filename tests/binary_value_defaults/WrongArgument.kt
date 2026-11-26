package binarydefaults.consumer
import binarydefaults.library.Key
import binarydefaults.library.OtherKey
import binarydefaults.library.top
fun invalid(): Key = top(key = OtherKey(1L))
