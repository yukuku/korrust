fun consume(): Boolean {
    val child = Child()
    child.name = "consumer"
    if (child.name != "consumer" || child.rename("inside") != "inside" || child.url != "url" || child.isReady || child.optional != null) return false
    val generic = GenericChild()
    generic.value = "binary"
    if (generic.value != "binary") return false
    val setter = KotlinSetter()
    setter.readOnly = "binary"
    return setter.recorded == "binary"
}
