package roots;
public final class Leaf extends Roots {
  public String toString() { return "leaf"; }
  public int hashCode() { return 97; }
  public boolean equals(Object other) { return this == other; }
}
