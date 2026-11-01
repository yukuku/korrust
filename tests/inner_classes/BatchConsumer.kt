package innerconsumer
import innerprobe.Outer.Inner as ImportedInner
import innerprobe.Outer
fun batchRead(inner: ImportedInner): Int = inner.read()
fun batchMake(outer: Outer): ImportedInner = outer.Inner(2)
