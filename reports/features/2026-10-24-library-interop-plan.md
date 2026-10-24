# Library interoperability milestone

User-approved scope, recorded 2026-10-24. This is a plan, not a support claim; dated executable evidence determines implemented scope and limitations. Use the pinned specification and Kotlin 2.3.21 compiler as described in [the reference policy](../../references/README.md).

1. Read dependencies directly from JAR files.
2. Decode Kotlin metadata for library classes, functions, and properties.
3. Resolve imported top-level declarations across files and modules.
4. Read Java generic signatures.
5. Support type parameters, substitution, and generic bounds.
6. Infer type arguments for generic calls.
7. Support generic subtyping, variance, and type projections.
8. Interpret library nullability annotations and platform types.
9. Resolve overloads using generic types and candidate specificity.
10. Resolve extension functions from compiled Kotlin libraries.
11. Resolve library properties and their accessors.
12. Support named and default arguments for imported Kotlin functions.
13. Represent and invoke function types.
14. Compile lambdas that capture local variables.
15. Infer lambda parameter and result types from their expected context.
16. Support bound and unbound callable references.
17. Resolve inherited members and dispatch interface calls.
18. Convert lambdas to Java single-method interfaces.
19. Inline function bodies from compiled Kotlin libraries.
20. Support non-local returns from inline lambdas.

Validation uses independently compiled Java and Kotlin dependency JARs, official Kotlin consumers, JVM verification, and unchanged AndroidBible sources where their dependencies and syntax are supported. Library APIs must resolve through reusable compiler rules.
