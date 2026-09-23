# Reference — Provider Descriptors

A technique step doesn't call a cloud SDK directly — it references a **provider descriptor** by
`service` and `operation`, and the host brokers the call (least-privilege content, NFR-SEC5). You
write descriptors as ordinary step bodies:

```yaml
body:
  type: call
  service: iam            # provider service
  operation: CreateAccessKey  # operation within the service
  params:                 # static or CEL-templated
    UserName: "$user"
```

- `service.operation` is the **capability key** the host grants for that step (e.g.
  `iam.CreateAccessKey`). A step can only invoke the capability it declared.
- `params` are resolved with the [expression/templating rules](cel-surface.md).
- The **revert** is itself a descriptor call (the inverse operation).

## What operations exist?

Available `service.operation`s are defined by the **provider adapter**. For AWS v1 the executor
implements `iam.CreateAccessKey` / `iam.DeleteAccessKey` and enumeration `iam.ListUsers` /
`iam.ListRoles`; more are added additively behind the seam (see the internal
[add-a-provider runbook](../../internal/extending/add-a-provider.md)). A technique may reference an
operation the adapter doesn't implement yet — it validates, previews, and runs on the Mock, and
executes live once the adapter grows that call.

## Enumeration vs. action descriptors

- **Enumeration** descriptors (used by `akumo enumerate --descriptor iam.ListUsers`) read the target;
  the adapter maps the response into graph assertions.
- **Action** descriptors (technique steps) perform read/mutating operations; mutating ones carry the
  technique's impact and require a revert.

> The declarative response-/result-mapping language is an internal detail today (the adapter maps in
> code); the authoring surface is the `service`/`operation`/`params` shown above.
