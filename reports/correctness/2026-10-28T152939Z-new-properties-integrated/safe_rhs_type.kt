import visibilityprobe.SafeTarget
fun bad(value: SafeTarget?) { value?.value = "bad" }
