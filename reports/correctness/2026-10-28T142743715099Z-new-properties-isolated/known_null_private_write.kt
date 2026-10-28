import visibilityprobe.PrivateDefault
fun bad() { val value: PrivateDefault? = null; value?.value = 1 }
