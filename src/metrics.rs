use std::collections::HashMap;

use chrono::{TimeZone, Timelike, Utc};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use surf::Client;
use tide::{Request, Response};

use crate::permissions::{has_permissions, Permissions};

#[derive(Deserialize, Debug)]
struct MetricsReq {
  start: i64,
  end: i64,
  step: i32,
  metrics: MetricsType,
}

#[derive(Deserialize, Debug)]
enum MetricsType {
  Cpu,
  Memory,
  Network,
  Disk,
}

#[derive(Deserialize, Debug)]
struct Metrics<T> {
  data: MetricsData<T>
}

#[derive(Deserialize, Debug)]
struct MetricsData<T> {
  result: Vec<MetricsTable<T>>,
}

#[derive(Deserialize, Debug)]
struct MetricsTable<T> {
  metric: T,
  values: Vec<Data>,
}

#[derive(Deserialize, Debug)]
struct Data {
  time: i64,
  value: String,
}

#[derive(Deserialize, Debug)]
struct CPU {
  cpu: String,
}

#[derive(Deserialize, Debug)]
struct Empty {}

#[derive(Serialize, Debug)]
struct MetricsRes {
  data: HashMap<String, Vec<(i64, f32)>>,
}

pub(crate) async fn metrics(mut req: Request<()>) -> tide::Result {
    if !has_permissions(&req, Permissions::Metrics as i32) {
        return Ok(tide::Response::new(403));
    }

    let MetricsReq { start, end, step, metrics } = req.body_json().await?;
    if start > end || step < 1 {
      return Ok(tide::Response::new(400));
    }

    let body = match metrics {
      MetricsType::Cpu => {
        let metrics = get_metrics::<Metrics<CPU>>("sum by (cpu) (irate(node_cpu_seconds_total{job=\"node\", mode!=\"idle\"}[30s])) * 100", start, end, step).await?;
        
        let mut cores: HashMap<String, Vec<(i64, f32)>> = HashMap::new();
        for core in metrics.data.result {
          cores.insert(core.metric.cpu, core.values.iter().map(|d| (d.time, d.value.parse::<f32>().unwrap())).collect());
        }
        
        let mut total: Vec<(i64, f32)> = Vec::new();
        for (_core, values) in cores.iter() {
          for (i, value) in values.iter().enumerate() {
            if i >= total.len() {
              total.push((value.0, value.1));
            } else {
              total[i].1 += value.1;
            }
          }
        }
        for value in total.iter_mut() {
          value.1 /= cores.len() as f32;
        }
        cores.insert("total".to_string(), total);
        let res_body = MetricsRes { data: cores };
        tide::Body::from_json(&res_body)?
      },
      MetricsType::Memory => {
        let metrics = get_metrics::<Metrics<Empty>>("100 * (1 - (node_memory_MemFree_bytes + node_memory_Cached_bytes) / node_memory_MemTotal_bytes)", start, end, step).await?;

        let mut data: HashMap<String, Vec<(i64, f32)>> = HashMap::new();
        data.insert("memory".to_string(), metrics.data.result[0].values.iter().map(|d| (d.time, d.value.parse::<f32>().unwrap())).collect());

        tide::Body::from_json(&MetricsRes { data })?
      },
      MetricsType::Network => {
        let incoming = get_metrics::<Metrics<Empty>>("irate(node_network_receive_bytes_total{device=\"eth0\"}[30s])", start, end, step).await?;
        let outgoing = get_metrics::<Metrics<Empty>>("irate(node_network_transmit_bytes_total{device=\"eth0\"}[30s])", start, end, step).await?;

        let mut data: HashMap<String, Vec<(i64, f32)>> = HashMap::new();
        data.insert("incoming".to_string(), incoming.data.result[0].values.iter().map(|d| (d.time, d.value.parse::<f32>().unwrap())).collect());
        data.insert("outgoing".to_string(), outgoing.data.result[0].values.iter().map(|d| (d.time, d.value.parse::<f32>().unwrap())).collect());

        tide::Body::from_json(&MetricsRes { data })?
      },
      MetricsType::Disk => {
        let free = get_metrics::<Metrics<Empty>>("node_filesystem_avail_bytes{mountpoint=\"/\"}", start, end, step).await?;
        let total = get_metrics::<Metrics<Empty>>("node_filesystem_size_bytes{mountpoint=\"/\"}", start, end, step).await?;

        let mut data: HashMap<String, Vec<(i64, f32)>> = HashMap::new();
        data.insert("free".to_string(), free.data.result[0].values.iter().map(|d| (d.time, d.value.parse::<f32>().unwrap())).collect());
        data.insert("total".to_string(), total.data.result[0].values.iter().map(|d| (d.time, d.value.parse::<f32>().unwrap())).collect());

        tide::Body::from_json(&MetricsRes { data })?
      },
    };

    let mut res = Response::new(200);
    res.set_body(body);
    Ok(res)
}

async fn get_metrics<T>(query: &str, start: i64, end: i64, step: i32) -> surf::Result<T> 
where T: for<'de> Deserialize<'de> {
  let client = Client::new();
  let start = Utc.timestamp_millis_opt(start).single().unwrap_or_default().with_second(0).unwrap_or_default().format("%Y-%m-%dT%H:%M:%SZ").to_string();
  let end = Utc.timestamp_millis_opt(end).single().unwrap_or_default().with_second(0).unwrap_or_default().format("%Y-%m-%dT%H:%M:%SZ").to_string();
  
  let encoded_query = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
  let url = format!("{}:9090/api/v1/query_range?query={}&start={}&end={}&step={}m", *crate::METRICS_HOST, encoded_query, start, end, step);
  
  let mut res = client.get(url).await?;
  
  Ok(res.body_json().await?)
}