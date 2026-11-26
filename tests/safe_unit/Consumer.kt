fun consume(): Boolean {
    val value = Effects()
    val missing: Unit? = safe(null)
    val present: Unit? = safe(value)
    val holder = UnitHolder(present)
    force(holder.value)
    val action: () -> Unit? = callback(value)
    return missing == null && present != null && holder.value != null && action() != null && generic(null) == null
}
