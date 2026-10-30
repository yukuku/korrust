fun consume(host: Host): String = with(host) { "O".decorated + "O".defaulted() }
fun masks(host: Host): Int = with(host) { 1.mask() + 1.mask(p31 = 100) }
fun companion(): String = with(Container) { "O".decorate() + "O".decorated }
fun top(): Int = "OK".topLength + 1.topMask()
fun update(host: Host): Int = with(host) { "O".amount = 5; 2.amount }
