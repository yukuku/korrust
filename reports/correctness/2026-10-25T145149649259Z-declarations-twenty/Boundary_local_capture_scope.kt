fun bad(): Int {
 fun capture(): Int = later
 val later = 1
 return capture()
}
