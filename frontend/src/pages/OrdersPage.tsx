import { useOrders } from "@/hooks/useApi";

export function OrdersPage() {
  const { data: orders } = useOrders();

  return (
    <div className="space-y-4">
      <h2 className="text-xl font-semibold">Orders</h2>
      <div className="rounded-lg border border-gray-800 bg-gray-950">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-gray-800 text-left text-gray-400">
              <th className="px-4 py-2">Platform</th>
              <th className="px-4 py-2">Outcome</th>
              <th className="px-4 py-2">Side</th>
              <th className="px-4 py-2">Type</th>
              <th className="px-4 py-2">Price Limit</th>
              <th className="px-4 py-2">Requested</th>
              <th className="px-4 py-2">Filled</th>
              <th className="px-4 py-2">Status</th>
            </tr>
          </thead>
          <tbody>
            {(orders || []).map((o: any) => (
              <tr key={o.id} className="border-b border-gray-800 hover:bg-gray-900">
                <td className="px-4 py-2">{o.platform}</td>
                <td className="px-4 py-2">{o.outcome}</td>
                <td className="px-4 py-2">{o.side}</td>
                <td className="px-4 py-2">{o.order_type}</td>
                <td className="px-4 py-2">{o.price_limit}</td>
                <td className="px-4 py-2">{o.requested_contracts}</td>
                <td className="px-4 py-2">{o.filled_contracts}</td>
                <td className="px-4 py-2">
                  <span className={`rounded px-2 py-0.5 text-xs ${
                    o.status === "filled" ? "bg-green-900 text-green-200" :
                    o.status === "pending" ? "bg-yellow-900 text-yellow-200" :
                    "bg-gray-800 text-gray-300"
                  }`}>
                    {o.status}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {(!orders || orders.length === 0) && (
          <p className="px-4 py-6 text-center text-gray-500">No orders found.</p>
        )}
      </div>
    </div>
  );
}
