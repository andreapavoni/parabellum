import { Link } from "@/components/Link";

export function HomePage() {
  return (
    <div class="mx-auto flex max-w-5xl flex-col gap-8 px-4 py-10">
      <section class="rounded-2xl border border-stone-200 bg-white p-8 shadow-sm">
        <h1 class="text-4xl font-black tracking-tight text-stone-800">Parabellum</h1>
        <p class="mt-4 max-w-3xl text-sm leading-7 text-stone-600">
          A modern, Rust-powered strategy game inspired by the classic Travian loop. The server now
          exposes a JSON API and this frontend is a lightweight Preact SPA layered over the
          existing game engine.
        </p>
        <div class="mt-6 flex gap-3">
          <Link to="/login" class="rounded bg-green-700 px-4 py-2 text-sm font-semibold text-white hover:bg-green-800">
            Login
          </Link>
          <Link to="/register" class="rounded border border-stone-300 px-4 py-2 text-sm font-semibold text-stone-700 hover:bg-stone-50">
            Register
          </Link>
        </div>
      </section>
    </div>
  );
}
