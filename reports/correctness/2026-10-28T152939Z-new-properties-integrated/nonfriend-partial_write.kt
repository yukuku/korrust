import visibilityprobe.PublicInternalSetter
fun bad(value: PublicInternalSetter) { value.value = 1 }
