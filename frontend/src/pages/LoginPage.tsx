import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useLogin } from "@/hooks/useApi";

export function LoginPage() {
  const [username, setUsername] = useState("admin");
  const [password, setPassword] = useState("");
  const login = useLogin();
  const navigate = useNavigate();

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    login.mutate(
      { username, password },
      {
        onSuccess: () => navigate("/"),
        onError: () => alert("Invalid credentials"),
      }
    );
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-gray-900">
      <form
        onSubmit={handleSubmit}
        className="w-full max-w-sm rounded-lg border border-gray-800 bg-gray-950 p-6 space-y-4"
      >
        <h2 className="text-xl font-semibold text-white">Arbitek Login</h2>
        <div>
          <label className="block text-sm text-gray-400">Username</label>
          <input
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            className="mt-1 w-full rounded border border-gray-700 bg-gray-900 px-3 py-2 text-sm text-gray-200"
          />
        </div>
        <div>
          <label className="block text-sm text-gray-400">Password</label>
          <input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="mt-1 w-full rounded border border-gray-700 bg-gray-900 px-3 py-2 text-sm text-gray-200"
          />
        </div>
        <button
          type="submit"
          disabled={login.isPending}
          className="w-full rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-500 disabled:opacity-50"
        >
          {login.isPending ? "Signing in..." : "Sign In"}
        </button>
      </form>
    </div>
  );
}
